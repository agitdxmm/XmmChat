package com.xmm.chat

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityEvent

/**
 * 无障碍服务。
 *
 * 它的作用是「看得见界面、点得动界面」，**不是**拿来跑 shell 的 ——
 * Android 的普通应用没有 shell 身份，无障碍也变不出一个来。
 * 真正的命令执行走 Shizuku（或 root），没有的话界面就显示
 * 「没有Shell权限的终端」。
 */
class XmmAccessibilityService : AccessibilityService() {

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // 预留：远程协助时读取当前窗口节点、模拟点击会用到这里。
    }

    override fun onInterrupt() {}

    override fun onDestroy() {
        instance = null
        super.onDestroy()
    }

    companion object {
        @Volatile
        var instance: XmmAccessibilityService? = null
            private set

        /** 服务是否处于已连接状态 */
        fun isRunning(): Boolean = instance != null
    }
}
