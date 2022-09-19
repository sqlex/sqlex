<div align="center">
    <img src="assets/logo.svg" style="width: 300px" alt="logo"/>
</div>

---

[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/sqlex/sqlex/build%20and%20release?style=flat-square)](https://github.com/sqlex/sqlex/actions/workflows/buildAndRelease.yml)
[![GitHub issues](https://img.shields.io/github/issues/sqlex/sqlex?style=flat-square)](https://github.com/sqlex/sqlex/issues)
[![Maven Central](https://img.shields.io/maven-central/v/me.danwi.sqlex/core?style=flat-square)](https://search.maven.org/search?q=me.danwi.sqlex)
[![Document](https://img.shields.io/badge/doc-reference-brightgreen?style=flat-square)](https://sqlex.github.io)
[![LICENSE](https://img.shields.io/github/license/sqlex/sqlex.svg?style=flat-square)](https://github.com/sqlex/sqlex/blob/master/LICENSE)

### SqlEx 是什么

SqlEx (SQL extension) 是一个简单的 DB helper.  
从实际应用角度出发, 解决编程语言(Java)和关系型数据库之间对于类型和结构认知不匹配的问题.  
主要思路是通过对数据库结构和 SQL 做语义分析, 依据分析得出的结果, 生成对应的结果类型. 提供`强类型安全`的编程体验.  
将大部分错误从`运行时`前推到`编译时`/从`编译时`前推到`编辑时`, 保证错误写法有提示, 错误写法无法编译通过, 能编译通过进运行环境的程序不会出现数据库结构/类型错误.

### 支持范围

- JDK >= 1.8
- Gradle >= 6
- Maven >= 3
- IDEA 版本 >= 2021.1
- 目前数据库只支持 MySQL

### IDEA 插件安装

SqlEx 的编辑体验**严重**依赖于 IDEA 插件, 插件提供了诸如 `智能提示`,`错误感知` 等功能.  
插件可以在 **IDEA 内置插件管理器**(搜索sqlex) 或者 [官方插件市场](https://plugins.jetbrains.com/plugin/19025-sqlex) 下载安装.

**!!强烈建议先安装 IDEA 插件!!**

### 构建系统配置

SqlEx 的编译任务依赖于构建系统, 目前支持 Gradle 和 Maven.  
当前中央仓库最新版本
[![Maven Central](https://img.shields.io/maven-central/v/me.danwi.sqlex/core?style=flat-square)](https://search.maven.org/search?q=me.danwi.sqlex)  
使用 IDEA 新建项目后, 可以在构建配置中引入 SqlEx 相关的插件和依赖.

#### Gradle

```groovy
plugin {
    id 'me.danwi.sqlex' version '{{最新版本号}}'
}

dependencies {
    implementation 'me.danwi.sqlex:core' //这里无须填写版本号,会由插件来自动配置
    implementation 'mysql:mysql-connector-java:8.0.29'
}
```

#### Maven

```xml
<dependencies>
    <dependency>
        <groupId>me.danwi.sqlex</groupId>
        <artifactId>core</artifactId>
        <version{{最新版本号}}</version>
    </dependency>
    <dependency>
        <groupId>mysql</groupId>
        <artifactId>mysql-connector-java</artifactId>
        <version>8.0.29</version>
    </dependency>
</dependencies>
<build>
    <plugins>
        <plugin>
            <groupId>me.danwi.sqlex</groupId>
            <artifactId>sqlex-maven-plugin</artifactId>
            <version>{{最新版本号}}</version>
            <executions>
                <execution>
                    <goals>
                        <goal>generate</goal>
                    </goals>
                </execution>
            </executions>
        </plugin>
    </plugins>
</build>
```

### 创建 SqlEx Repository

**Repository** 是 SqlEx 中的一个重要概念.  
从感性角度来理解, 可以认为一个 Repository 对应着一个数据库.  
其包含了 **编译配置** / **数据库结构的描述** / **针对数据的 CRUD 方法定义** 等, Repository 拥有自己的 ID, 以 java 包的方式描述, 称之为 **根包名**.

以 gradle 工程为例:

1. 在 src/main 下新建名为 sqlex 的文件夹, 然后在 sqlex 夹下建立 sqlex.sqlc 配置文件.  
2. 文件夹和配置文件建立后, 修改 sqlex.sqlc 配置文件, 修改根包名, 我们这里修改为`me.danwi.sqlex.example.dao`, 然后点击右上角的 + 图标, 将其导入编辑器解析.  
3. 如果配置文件没有错误, 编辑器成功导入. 那么 Repository 文件夹将会被标记为源码文件夹(蓝色), 同时 **SqlEx Repository 工具窗** 将出现你导入的 Repository 信息.   

至此, Repository 已经创建完毕.

### 设计数据库结构

Repository 建立完毕后, 可以针对该 Repository 做 Schema 描述.  
Schema 信息以迁移脚本的方式表达, 兼顾 **数据库版本迁移** 和 **结构描述** 功能, 文件拓展名为 `sqls`.

1. 在根包目录下创建子目录用于存放 Schema 信息, 我们这里是 `me.danwi.sqlex.example.dao.migrations`, 创建数据库版本 0 的迁移脚本.  
   文件名: `000-person-table.sqls`.  
   全路径: `src/main/sqlex/me/danwi/sqlex/example/dao/migrations/000-person-table.sqls`

```sql
create table person(
    id integer auto_increment primary key,
    name varchar(255) not null
);
```

2. 文件建立完成后, 可以点击右上角的 "同步" 按钮来更新索引信息.  

3. 同步完成后, **Database 工具窗** 将出现对应的数据库信息.  

自此, 数据库结构的描述初步完成.

### 编写数据访问方法

数据访问方法, 使用自定义的语言(`SqlEx Method`)编写, 文件拓展名为 `sqlm`;

我们建立一个针对`Person`业务的数据访问方法文件, `PersonDao.sqlm`, 目录为 `me/danwi/sqlex/example/dao/PersonDao.sqlm`

```
findAll() {
    select *
    from person
}
```

很容易理解, 方法名称叫 `findAll`, SQL 为 `select * from perosn`, 该方法的作用就是查询出 `person` 表中所有的行.

### 在 Java 中使用 DAO 方法

经过上面的步骤, SqlEx 部分的代码已经全部完成. 现在我们编写 MainClass, 来调用 SqlEx 写的方法, 做数据库访问.

```java
package me.danwi.sqlex.example;

import me.danwi.sqlex.core.DaoFactory;
import me.danwi.sqlex.example.dao.PersonDao;
import me.danwi.sqlex.example.dao.Repository;

import java.util.List;

public class Main {
    public static void main(String[] args) {
        //新建dao factory
        DaoFactory factory = new DaoFactory(
                "jdbc:mysql://localhost:3306/sqlex",
                "root", "password",
                Repository.class //自动生成的Repository类
        );
        //迁移数据库版本
        factory.migrate();
        //检查数据库结构的一致性
        factory.check();

        //获取Dao
        PersonDao personDao = factory.getInstance(PersonDao.class); //自动生成的PersonDao类

        //查询数据
        List<PersonDao.FindAllResult> results = personDao.findAll(); //自动生成的FindAllResult结果类
        if (!results.isEmpty()) {
            System.out.println(results.get(0).getName());
        }
    }
}
```

如上代码, 其中 `me.danwi.sqlex.example.dao.Repository` 类型代表了该 Repository, 其上有 Repository 的元信息. 使用该类型作为参数创建一个 `DaoFactory`. 可以使用该工厂去实例化数据访问对象. 我们这里创建了一个 `PersonDao` 的实例, 然后调用我们自己编写的 `findAll` 方法, 获取到一个 `PersonDao.FindAllResult` 的列表, 这个列表就是查询结果.

上述的 `Repository`/`PersonDao`/`PersonDao.FindAllResult` 都由框架自动生成, 其中 `FindAllResult` 类的属性, 通过结合数据库结构定义, 然后分析 SQL 语句得出.

### Fluent API(单表 API)

`SqlEx Method`做数据库访问的方式, 比较适合**复杂的 SQL 查询**(比如: 多表 join, group by 等). 而简单的` 增删改查` 可以通过 `Fluent API` 来操作.

```java
//获取表操作对象的实例
PersonTable personTable = factory.getInstance(PersonTable.class);

//select * from person
List<Person> persons = personTable.select().find();

//select * from person where name like 'sqlex'
persons = personTable.select().where(PersonTable.Name.like(Expression.arg("sqlex"))).find();

//select * from person where id = ?
Person person = personTable.findById(1);

//select * from person where id > 1
persons = personTable.select().where(PersonTable.Name.gt(Expression.arg(1))).find();

//update person set name = 'sqlex' where id = 1
long affectedRows = personTable.update().setName("sqlex").where(PersonTable.Id.eq(Expression.arg(1))).execute();

//delete... insert...

```

针对 `kotlin` 语言, SqlEx 提供了部分语法糖, 能让 `where` 方法中的条件表达式写起来更加优雅.

### 数据库结构变更

随着业务的变化, 数据库的结构可能会产生变化, 比如新增了 `部门` 这个实体. 我们可以通过如下几步完成该变更:

1. 增加部门表, Person 表中添加部门 id.  
   新建一个版本 1 的数据库迁移文件, `001-add-department.sqls`, 然后刷新 SqlEx 索引.

```sql
# 创建department表
create table department
(
    id   integer auto_increment primary key,
    name varchar(255) not null
);

# 给person表添加部门ID
alter table person
    add column depart_id integer not null;
```

2. 编写业务 SQL  
   假设我们有一个业务需要统计各个部门的人数, 显而易见, 其 SQL 为:

```sql
select d.name, count(1) as count
from person p
         left join department d on p.depart_id = d.id
group by d.id
```

返回结果有两个字段, 一个 name(字符串), 一个 count(数字).  
我们将其通过`SqlEx Method`来实现, 继续在 `PersonDao.sqlm` 增加一个方法:

```
countByDepartment() {
    select d.name, count(1) as count
    from person p
             left join department d on p.depart_id = d.id
    group by d.id
}
```

然后在 Java 中调用

```java
List<PersonDao.CountByDepartmentResult> countResult = personDao.countByDepartment();
PersonDao.CountByDepartmentResult result = countResult.get(0);
String name = result.getName(); //name字段, 类型为String
Long count = result.getCount(); //count字段, 类型为Long
```

如上代码所示, 框架会自动生成 `PersonDao.CountByDepartmentResult` 实体类, 其属性的名称/类型均与 `SQL` 一一对应.

随着后面业务的继续变更, 数据库的结构也会继续产生变化, 但是不管如何变化. 在程序编译时, SqlEx 会根据最新的数据库结构信息来分析所有的 SQL. 保证字段的存在和类型的安全.  
当出现如下破坏性变更时:

- 删除了某个字段, 但是 SQL / Java 中还在使用
- 修改了字段的类型, 但是 Java 还以旧的认知在使用

程序编译时会发生编译错误, 只能挨个修复完毕才能保证编译通过, 最大程度保证了程序的运行时安全.  
再也不会出现一个复杂 SQL 需要写一个实体类来手动映射(或者写一个大而全的 Fat Class), 也不怕删改字段了. 全面提升对于数据库的掌控程度.

### 总结

通过上面一个 "无用且蛋疼" 的例子, 简单介绍了 SqlEx 的设计.

你可以根据自己实际的项目需求, 创建多个迁移脚本(如 002-add-age-to-person.sqls), 或者创建一个复杂的 `join`/`groupby` 查询. 体验一下 SqlEx 框架自动(`分析`/`生成`/`强类型安全`)的魅力.

也可以访问 [Gradle Example(更全面)](https://github.com/sqlex/gradle-example), [Maven Example](https://github.com/sqlex/maven-example) 了解 SqlEx 的基本用法. 另外框架也对 `SpringBoot` 做了集成, 可以访问 [Spring Example](https://github.com/sqlex/spring-example) 了解具体集成的方法.

### 关于
本项目中MySQL相关的语法/逻辑计划分析来源于 [TiDB](https://github.com/pingcap/tidb) 项目  

另外部分代码的实现上参考了如下项目:
- [ktorm](https://github.com/kotlin-orm/ktorm)

再次特别感谢上述项目的开发者对开源事业做出的贡献.
