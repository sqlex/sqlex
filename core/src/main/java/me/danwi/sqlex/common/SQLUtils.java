package me.danwi.sqlex.common;

import org.antlr.v4.runtime.Parser;
import org.antlr.v4.runtime.ParserRuleContext;
import org.antlr.v4.runtime.atn.PredictionMode;
import org.antlr.v4.runtime.misc.ParseCancellationException;
import org.antlr.v4.runtime.tree.AbstractParseTreeVisitor;
import org.antlr.v4.runtime.tree.ErrorNode;
import org.antlr.v4.runtime.tree.ParseTree;
import org.antlr.v4.runtime.tree.RuleNode;
import org.apache.shardingsphere.sql.parser.api.parser.SQLParser;
import org.apache.shardingsphere.sql.parser.autogen.MySQLStatementParser;
import org.apache.shardingsphere.sql.parser.core.ParseASTNode;
import org.apache.shardingsphere.sql.parser.core.SQLParserFactory;
import org.apache.shardingsphere.sql.parser.exception.SQLParsingException;
import org.apache.shardingsphere.sql.parser.mysql.parser.MySQLParserFacade;

import java.util.*;

/**
 * SQL辅助工具
 */
public class SQLUtils {
    private static final MySQLParserFacade mysqlParserFacade = new MySQLParserFacade();

    //数据库名遍历器
    private static class DatabaseNameVisitor extends AbstractParseTreeVisitor<Object> {
        private final Map<String, String> mapping;
        private final Set<StringUtils.ReplaceInfo> replaces = new HashSet<>();

        DatabaseNameVisitor(Map<String, String> mapping) {
            this.mapping = mapping;
        }

        @Override
        public Object visit(ParseTree tree) {
            if (tree instanceof ParserRuleContext)
                visit((ParserRuleContext) tree);
            return super.visit(tree);
        }

        @Override
        public Object visitChildren(RuleNode node) {
            if (node instanceof ParserRuleContext)
                visit((ParserRuleContext) node);
            return super.visitChildren(node);
        }

        public void visit(ParserRuleContext ctx) {
            if (ctx instanceof MySQLStatementParser.TableFactorContext) {
                MySQLStatementParser.TableFactorContext tableFactorContext = (MySQLStatementParser.TableFactorContext) ctx;
                if (tableFactorContext.tableName() != null) {
                    if (tableFactorContext.tableName().owner() != null) {
                        MySQLStatementParser.OwnerContext owner = tableFactorContext.tableName().owner();
                        String databaseName = mapping.get(owner.getText());
                        if (databaseName != null) {
                            replaces.add(new StringUtils.ReplaceInfo(owner.getStart().getStartIndex(), owner.getStop().getStopIndex() + 1, databaseName));
                        }
                    }
                }
            } else if (ctx instanceof MySQLStatementParser.ColumnRefContext) {
                MySQLStatementParser.ColumnRefContext columnRefContext = (MySQLStatementParser.ColumnRefContext) ctx;
                List<MySQLStatementParser.IdentifierContext> identifiers = columnRefContext.identifier();
                if (identifiers.size() == 3) {
                    MySQLStatementParser.IdentifierContext schemaID = identifiers.get(0);
                    String databaseName = mapping.get(schemaID.getText());
                    if (databaseName != null) {
                        replaces.add(new StringUtils.ReplaceInfo(schemaID.getStart().getStartIndex(), schemaID.getStop().getStopIndex() + 1, databaseName));
                    }
                }
            }
        }
    }

    //替换缓存
    private static final LRUCache<String, String> replaceCache = new LRUCache<>(500);

    /**
     * 根据mapping将SQL中的数据库名做重映射
     * TODO: 不稳定版本,可能会出现替换错误
     *
     * @param sql     SQL语句
     * @param mapping 数据库名映射
     * @return 返回修改了数据库名后的SQL
     */
    public static String replaceDatabaseName(String sql, Map<String, String> mapping) {
        String key = sql + "." + mapping.hashCode();
        String result = replaceCache.get(key);
        if (result == null) {
            //解析为AST
            ParseASTNode ast = parse(sql);
            //遍历解析出需要替换的位置
            DatabaseNameVisitor databaseNameVisitor = new DatabaseNameVisitor(mapping);
            ast.getRootNode().accept(databaseNameVisitor);
            //替换字符串的位置
            result = StringUtils.replace(sql, new ArrayList<>(databaseNameVisitor.replaces));
            //存入缓存
            replaceCache.set(key, result);
        }
        return result;
    }

    /**
     * 将多条语句做拆分
     *
     * @param script SQL脚本
     * @return 单个SQL语句集合
     */
    public static List<String> splitStatements(String script) {
        script = script.trim();
        LinkedList<String> result = new LinkedList<>();
        while (true) {
            //解析SQL
            ParseASTNode ast = parse(script);
            if (ast.getRootNode() instanceof ParserRuleContext && ast.getRootNode().getParent() != null) {
                if (ast.getRootNode().getParent() instanceof ParserRuleContext) {
                    //释放语句
                    result.add(script.substring(
                            ((ParserRuleContext) ast.getRootNode()).getStart().getStartIndex(),
                            ((ParserRuleContext) ast.getRootNode()).getStop().getStopIndex() + 1
                    ));
                    //判断是否终结
                    ParserRuleContext ctx = (ParserRuleContext) ast.getRootNode().getParent();
                    if (ctx.getStop().getStopIndex() + 1 >= script.length())
                        break;
                    script = script.substring(ctx.getStop().getStopIndex() + 1);
                }
            }
        }
        return result;
    }

    //解析SQL
    private static ParseASTNode parse(String sql) {
        ParseASTNode result = twoPhaseParse(sql);
        if (result.getRootNode() instanceof ErrorNode) {
            throw new SQLParsingException("Unsupported SQL of `%s`", sql);
        } else {
            return result;
        }
    }

    private static ParseASTNode twoPhaseParse(String sql) {
        SQLParser sqlParser = SQLParserFactory.newInstance(sql, mysqlParserFacade.getLexerClass(), mysqlParserFacade.getParserClass());
        try {
            ((Parser) sqlParser).getInterpreter().setPredictionMode(PredictionMode.SLL);
            return (ParseASTNode) sqlParser.parse();
        } catch (ParseCancellationException var7) {
            ((Parser) sqlParser).reset();
            ((Parser) sqlParser).getInterpreter().setPredictionMode(PredictionMode.LL);
            try {
                return (ParseASTNode) sqlParser.parse();
            } catch (ParseCancellationException var6) {
                throw new SQLParsingException("You have an error in your SQL syntax");
            }
        }
    }
}
