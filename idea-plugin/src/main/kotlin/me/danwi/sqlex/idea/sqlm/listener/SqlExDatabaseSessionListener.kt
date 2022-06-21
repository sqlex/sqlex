package me.danwi.sqlex.idea.sqlm.listener

import com.intellij.database.console.client.VisibleDatabaseSessionClient
import com.intellij.database.console.session.DatabaseSession
import com.intellij.database.console.session.DatabaseSessionStateListener
import com.intellij.openapi.project.Project
import com.intellij.ui.EditorNotifications

class SqlExDatabaseSessionListener(private val project: Project) : DatabaseSessionStateListener {
    override fun clientAttached(client: VisibleDatabaseSessionClient) {
        EditorNotifications.getInstance(project).updateAllNotifications()
    }

    override fun clientDetached(client: VisibleDatabaseSessionClient) {
        EditorNotifications.getInstance(project).updateAllNotifications()
    }

    override fun clientReattached(
        client: VisibleDatabaseSessionClient,
        session1: DatabaseSession,
        session2: DatabaseSession
    ) {
        EditorNotifications.getInstance(project).updateAllNotifications()
    }

    override fun connected(session: DatabaseSession) {}

    override fun disconnected(session: DatabaseSession) {}

    override fun stateChanged(event: DatabaseSessionStateListener.ChangeEvent) {}

    override fun renamed(session: DatabaseSession) {}
}