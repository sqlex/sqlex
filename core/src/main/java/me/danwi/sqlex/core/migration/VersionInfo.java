package me.danwi.sqlex.core.migration;

import me.danwi.sqlex.core.annotation.entity.SqlExColumnName;

/**
 * 版本表信息
 */
public class VersionInfo {
    private String rootPackage;
    private Integer version;
    private Boolean canMigrate;

    public String getRootPackage() {
        return rootPackage;
    }

    @SqlExColumnName("package")
    public void setRootPackage(String rootPackage) {
        this.rootPackage = rootPackage;
    }

    public Integer getVersion() {
        return version;
    }

    @SqlExColumnName("version")
    public void setVersion(Integer version) {
        this.version = version;
    }

    public Boolean getCanMigrate() {
        return canMigrate;
    }

    @SqlExColumnName("can_migrate")
    public void setCanMigrate(Boolean canMigrate) {
        this.canMigrate = canMigrate;
    }
}

