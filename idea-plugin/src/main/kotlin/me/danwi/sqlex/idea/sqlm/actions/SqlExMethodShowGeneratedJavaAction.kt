package me.danwi.sqlex.idea.sqlm.actions

import com.intellij.ide.highlighter.HighlighterFactory
import com.intellij.ide.highlighter.JavaFileType
import com.intellij.lang.java.JavaLanguage
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.editor.EditorFactory
import com.intellij.openapi.editor.ex.EditorEx
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.popup.JBPopupFactory
import com.intellij.psi.PsiFile
import com.intellij.psi.PsiFileFactory
import com.intellij.psi.codeStyle.CodeStyleManager
import com.intellij.ui.popup.PopupPositionManager
import com.intellij.util.DocumentUtil
import me.danwi.sqlex.idea.util.extension.isSqlExMethod
import me.danwi.sqlex.idea.util.extension.showNotification
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService
import java.awt.BorderLayout
import javax.swing.JPanel

class SqlExMethodShowGeneratedJavaAction : AnAction() {
    override fun update(event: AnActionEvent) {
        event.presentation.isVisible = event.getData(CommonDataKeys.VIRTUAL_FILE).isSqlExMethod
    }

    override fun actionPerformed(event: AnActionEvent) {
        val project = event.getData(CommonDataKeys.PROJECT) ?: return
        val methodFile = event.getData(CommonDataKeys.VIRTUAL_FILE) ?: return
        val editor = event.getData(CommonDataKeys.EDITOR) ?: return

        try {
            val service = methodFile.sqlexRepositoryService ?: throw Exception("该文件不存在对应的索引服务,请尝试重建索引")
            if (!service.isValid)
                throw Exception("索引已经过期,请先重建索引")

            //生成java文件
            val javaFile = service.generateJavaFile(methodFile) ?: throw Exception("Java文件生成失败")
            //解析为psi
            val psiJavaFile = PsiFileFactory.getInstance(project)
                .createFileFromText(JavaLanguage.INSTANCE, javaFile.source)
            //格式化
            DocumentUtil.writeInRunUndoTransparentAction {
                CodeStyleManager.getInstance(project).reformat(psiJavaFile)
            }
            //创建编辑器视图
            val editorView = GeneratedJavaFileViewComponent(project, psiJavaFile)
            //创建弹出框
            val popup = JBPopupFactory.getInstance().createComponentPopupBuilder(editorView, editorView)
                .setProject(project)
                .setResizable(true)
                .setMovable(true)
                .setRequestFocus(true)
                .setTitle("生成文件 ${javaFile.relativePath} 的内容")
                .createPopup()
            //调整位置
            PopupPositionManager.positionPopupInBestPosition(popup, editor, null)
        } catch (e: Exception) {
            project.showNotification(e.message ?: "未知错误", NotificationType.WARNING)
        }
    }
}

private class GeneratedJavaFileViewComponent(project: Project, javaFile: PsiFile) : JPanel(BorderLayout()) {
    private val editor: EditorEx

    init {
        //新建编辑器和文档
        val editorFactory = EditorFactory.getInstance()
        val document = editorFactory.createDocument(javaFile.text)
        document.setReadOnly(true)
        editor = editorFactory.createEditor(document) as EditorEx
        //设置编辑器的样式
        with(editor.settings) {
            isLineMarkerAreaShown = false
            isIndentGuidesShown = false
            isAutoCodeFoldingEnabled = true
        }
        //语法高亮
        editor.highlighter = HighlighterFactory.createHighlighter(project, JavaFileType.INSTANCE)
        //添加到组件树
        add(editor.component)
    }

    override fun removeNotify() {
        super.removeNotify()

        EditorFactory.getInstance().releaseEditor(editor)
    }
}