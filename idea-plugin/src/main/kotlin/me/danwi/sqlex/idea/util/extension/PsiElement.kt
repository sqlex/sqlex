package me.danwi.sqlex.idea.util.extension

import com.intellij.lang.injection.InjectedLanguageManager
import com.intellij.psi.PsiElement
import com.intellij.psi.util.PsiTreeUtil

inline fun <reified T : PsiElement> PsiElement.childrenOf(): Collection<T> {
    return PsiTreeUtil.findChildrenOfType(this, T::class.java)
}

inline fun <reified T : PsiElement> PsiElement.parentOf(): T? {
    return PsiTreeUtil.getParentOfType(this, T::class.java)
}

inline fun <reified T> PsiElement.injectedOf(): T? {
    val files = mutableListOf<T>()
    InjectedLanguageManager.getInstance(project)
        .enumerate(this) { file, _ ->
            if (file is T)
                files.add(file)
        }
    return files.firstOrNull()
}