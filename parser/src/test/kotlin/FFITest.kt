import me.danwi.sqlex.parser.ffi.ffiInvoke
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.*

class FFITest {
    @Test
    fun name() {
        val name: String = ffiInvoke("SystemAPI", "Name")
        assertEquals("SqlEx Native", name)
    }
}