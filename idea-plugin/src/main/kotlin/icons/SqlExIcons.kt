package icons

import com.intellij.icons.AllIcons
import com.intellij.openapi.util.IconLoader
import com.intellij.ui.LayeredIcon
import com.intellij.util.IconUtil

object SqlExIcons {
    private val Icon = IconLoader.getIcon("icons/sqlex.svg", javaClass)

    private val IconX12 = IconUtil.resizeSquared(Icon, 12) //Editor gutter
    private val IconX13 = IconUtil.resizeSquared(Icon, 13) //Tool window
    private val IconX16 = IconUtil.resizeSquared(Icon, 16) //Node,Action,Filetype

    @JvmField
    val ConfigFile = IconUtil.addText(IconX16, "C")

    @JvmField
    val SchemaFile = IconUtil.addText(IconX16, "S")

    @JvmField
    val MethodFile = IconUtil.addText(IconX16, "M")

    @JvmField
    val Method = MethodFile

    @JvmField
    val ShowSourceAction = IconUtil.addText(IconX16, "Java")

    @JvmField
    val ToolWindow = IconUtil.desaturate(IconX13)

    val RefreshAction by lazy {
        val layeredIcon = LayeredIcon(2)
        layeredIcon.setIcon(IconX16, 0)
        layeredIcon.setIcon(IconUtil.scale(AllIcons.Actions.BuildLoadChanges, 0.8), 1, 4)
        layeredIcon
    }
}