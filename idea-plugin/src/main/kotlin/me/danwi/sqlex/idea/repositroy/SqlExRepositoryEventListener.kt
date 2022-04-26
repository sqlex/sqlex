package me.danwi.sqlex.idea.repositroy

import com.intellij.execution.ui.ConsoleViewContentType
import com.intellij.util.messages.Topic

//RepositoryService变更监听
interface SqlExRepositoryEventListener {
    companion object {
        @JvmStatic
        val REPOSITORY_SERVICE_TOPIC =
            Topic.create("SqlEx repository service event", SqlExRepositoryEventListener::class.java)
    }

    fun created(newRepositoryService: SqlExRepositoryService) {}
    fun removed(oldRepositoryService: SqlExRepositoryService) {}
    fun beforeRefresh(repositoryService: SqlExRepositoryService) {}
    fun afterRefresh(repositoryService: SqlExRepositoryService) {}
    fun validChanged(repositoryService: SqlExRepositoryService, isValid: Boolean) {}
    fun clearOutput(repositoryService: SqlExRepositoryService) {}
    fun output(repositoryService: SqlExRepositoryService, text: String, type: ConsoleViewContentType) {}
}