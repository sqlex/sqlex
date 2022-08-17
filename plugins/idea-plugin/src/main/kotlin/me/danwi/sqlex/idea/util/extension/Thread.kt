package me.danwi.sqlex.idea.util.extension

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.application.ModalityState
import com.intellij.openapi.util.Computable
import java.util.concurrent.Executors

inline fun <T> runWriteAction(crossinline runnable: () -> T): T {
    return ApplicationManager.getApplication().runWriteAction(Computable { runnable() })
}

inline fun <T> runReadAction(crossinline runnable: () -> T): T {
    return ApplicationManager.getApplication().runReadAction(Computable { runnable() })
}

inline fun invokeLater(modalityState: ModalityState? = null, crossinline runnable: () -> Unit) {
    ApplicationManager.getApplication()
        .invokeLater({ runnable() }, modalityState ?: ModalityState.defaultModalityState())
}

private val backgroundThreadPool = Executors.newFixedThreadPool(4)

fun runInThread(runnable: () -> Unit) {
    backgroundThreadPool.submit(runnable)
}