package me.danwi.sqlex.idea.listener

import com.intellij.util.messages.Topic
import me.danwi.sqlex.idea.repositroy.SqlExRepositoryService

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
}