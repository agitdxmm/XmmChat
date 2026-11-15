package com.xmm.chat

import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.ParcelFileDescriptor
import android.provider.Settings
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import moe.shizuku.server.IShizukuService
import rikka.shizuku.Shizuku

@InvokeArg
class RunArgs {
    var cmd: String = ""
    var timeoutMs: Long? = null
}

/**
 * 安卓端的执行能力。
 *
 * 三种后端，按优先级：
 *   1. shizuku —— Shizuku 在跑且已授权，可以拿到 shell 级身份
 *   2. root    —— 设备已 root
 *   3. none    —— 都没有，只能显示「没有Shell权限的终端」
 *
 * 无障碍服务单独上报（a11y 字段），用于远程协助的界面操作，
 * 它不能执行 shell 命令。
 */
@TauriPlugin
class XmmShellPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val SHIZUKU_REQ = 4213
    }

    private fun shizukuReady(): Boolean = try {
        Shizuku.pingBinder() &&
            Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED
    } catch (t: Throwable) {
        false
    }

    private fun rootReady(): Boolean = try {
        val p = Runtime.getRuntime().exec(arrayOf("su", "-c", "id"))
        val out = p.inputStream.bufferedReader().readText()
        p.waitFor()
        out.contains("uid=0")
    } catch (t: Throwable) {
        false
    }

    @Command
    fun probe(invoke: Invoke) {
        val shizuku = shizukuReady()
        val root = if (shizuku) false else rootReady()
        val a11y = XmmAccessibilityService.isRunning()

        val backend = when {
            shizuku -> "shizuku"
            root -> "root"
            a11y -> "accessibility"
            else -> "none"
        }
        val detail = when (backend) {
            "shizuku" -> "Shizuku · shell 身份"
            "root" -> "root · shell 身份"
            "accessibility" -> "无障碍已开启 · 无 shell"
            else -> "没有Shell权限的终端"
        }

        val ret = JSObject()
        ret.put("backend", backend)
        ret.put("detail", detail)
        ret.put("shizuku", shizuku)
        ret.put("root", root)
        ret.put("a11y", a11y)
        invoke.resolve(ret)
    }

    @Command
    fun run(invoke: Invoke) {
        val args = try {
            invoke.parseArgs(RunArgs::class.java)
        } catch (t: Throwable) {
            invoke.reject("参数无法解析")
            return
        }

        val cmd = args.cmd.trim()
        if (cmd.isEmpty()) {
            invoke.reject("命令为空")
            return
        }

        val shizuku = shizukuReady()
        val root = if (shizuku) false else rootReady()
        if (!shizuku && !root) {
            invoke.reject("没有Shell权限的终端")
            return
        }

        val timeout = (args.timeoutMs ?: 20_000L).coerceIn(1_000L, 120_000L)

        Thread {
            try {
                val outText = StringBuilder()
                val errText = StringBuilder()
                var exit = -1
                var timedOut = false
                val deadline = System.currentTimeMillis() + timeout

                if (shizuku) {
                    // Shizuku 13 起没有公开的 newProcess 静态方法（Shell 类也不存在），
                    // 必须直接拿 AIDL 服务接口调用。
                    val svc = IShizukuService.Stub.asInterface(Shizuku.getBinder())
                    val proc = svc.newProcess(arrayOf("sh", "-c", cmd), null, null)

                    val t1 = Thread {
                        runCatching {
                            ParcelFileDescriptor.AutoCloseInputStream(proc.inputStream)
                                .bufferedReader().forEachLine { outText.appendLine(it) }
                        }
                    }
                    val t2 = Thread {
                        runCatching {
                            ParcelFileDescriptor.AutoCloseInputStream(proc.errorStream)
                                .bufferedReader().forEachLine { errText.appendLine(it) }
                        }
                    }
                    t1.start(); t2.start()

                    while (System.currentTimeMillis() < deadline) {
                        if (!proc.alive()) {
                            exit = proc.exitValue()
                            break
                        }
                        Thread.sleep(40)
                    }
                    if (exit == -1) {
                        timedOut = true
                        runCatching { proc.destroy() }
                    }
                    t1.join(500); t2.join(500)
                } else {
                    val proc = Runtime.getRuntime().exec(arrayOf("su", "-c", cmd))

                    val t1 = Thread {
                        runCatching { proc.inputStream.bufferedReader().forEachLine { outText.appendLine(it) } }
                    }
                    val t2 = Thread {
                        runCatching { proc.errorStream.bufferedReader().forEachLine { errText.appendLine(it) } }
                    }
                    t1.start(); t2.start()

                    while (System.currentTimeMillis() < deadline) {
                        try {
                            exit = proc.exitValue()
                            break
                        } catch (e: IllegalThreadStateException) {
                            Thread.sleep(40)
                        }
                    }
                    if (exit == -1) {
                        timedOut = true
                        runCatching { proc.destroyForcibly() }
                    }
                    t1.join(500); t2.join(500)
                }

                val ret = JSObject()
                ret.put("code", if (timedOut) -1 else exit)
                ret.put("stdout", outText.toString())
                ret.put("stderr", errText.toString())
                ret.put("timed_out", timedOut)

                activity.runOnUiThread { invoke.resolve(ret) }
            } catch (t: Throwable) {
                val msg = t.message ?: "执行失败"
                activity.runOnUiThread { invoke.reject(msg) }
            }
        }.start()
    }

    @Command
    fun openA11ySettings(invoke: Invoke) {
        try {
            val i = Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            activity.startActivity(i)
            invoke.resolve()
        } catch (t: Throwable) {
            invoke.reject(t.message ?: "无法打开无障碍设置")
        }
    }

    @Command
    fun requestShizuku(invoke: Invoke) {
        try {
            if (!Shizuku.pingBinder()) {
                invoke.reject("Shizuku 没有运行")
                return
            }
            if (Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED) {
                invoke.resolve()
                return
            }
            Shizuku.requestPermission(SHIZUKU_REQ)
            invoke.resolve()
        } catch (t: Throwable) {
            invoke.reject(t.message ?: "Shizuku 请求失败")
        }
    }
}
