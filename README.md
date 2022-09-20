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

### 特性一览

- 自带数据库版本管理(类似 flyway)
- 提供`sqlm`语言编写 SQL, 能自动生成结果类
- 根据数据库结构自动生成 Fluent API
- 提供 IDEA 插件, 提升开发体验
- 与 SpringBoot 良好集成

### SqlEx Method 语言

`sqlm`是另一种"写 Java"的方式, 专用于编写复杂的数据库访问方法. 能自动分析 SQL 并生成对应的结果类型.

动画演示(加载有点慢):

![image](assets/sqlm.gif)

### Fluent API

`SqlEx` 会根据数据库信息, 准备好对应的`表操作对象`(无须自己手动定义实体), 提供 `Fluent API` 来做数据库做简单操作.

动画演示(加载有点慢):

![fluent api](assets/fluent-api.gif)

### 更多

详细文档请访问 [SqlEx 文档](https://sqlex.github.io)

### 关于

本项目中 MySQL 相关的语法/逻辑计划分析来源于 [TiDB](https://github.com/pingcap/tidb) 项目

另外部分代码的实现上参考了如下项目:

- [ktorm](https://github.com/kotlin-orm/ktorm)

再次特别感谢上述项目的开发者对开源事业做出的贡献.
