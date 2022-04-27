package me.danwi.sqlex.core.migration;

public class Migration {
    private final int version;
    private final String[] scripts;

    public Migration(int version, String[] scripts) {
        this.version = version;
        this.scripts = scripts;
    }

    public int getVersion() {
        return version;
    }

    public String[] getScripts() {
        return scripts;
    }
}
