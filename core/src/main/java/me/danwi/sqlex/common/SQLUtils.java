package me.danwi.sqlex.common;

import org.antlr.v4.runtime.Parser;
import org.antlr.v4.runtime.ParserRuleContext;
import org.antlr.v4.runtime.atn.PredictionMode;
import org.antlr.v4.runtime.misc.ParseCancellationException;
import org.antlr.v4.runtime.tree.ErrorNode;
import org.antlr.v4.runtime.tree.ParseTree;
import org.apache.shardingsphere.sql.parser.api.parser.SQLParser;
import org.apache.shardingsphere.sql.parser.api.visitor.ASTNode;
import org.apache.shardingsphere.sql.parser.core.ParseASTNode;
import org.apache.shardingsphere.sql.parser.core.SQLParserFactory;
import org.apache.shardingsphere.sql.parser.exception.SQLParsingException;
import org.apache.shardingsphere.sql.parser.mysql.parser.MySQLParserFacade;
import org.apache.shardingsphere.sql.parser.mysql.visitor.statement.impl.MySQLStatementSQLVisitor;
import org.apache.shardingsphere.sql.parser.sql.common.segment.generic.OwnerAvailable;
import org.apache.shardingsphere.sql.parser.sql.common.segment.generic.OwnerSegment;

import java.util.*;

/**
 * SQL辅助工具
 */
public class SQLUtils {
    private static final MySQLParserFacade mysqlParserFacade = new MySQLParserFacade();

    //数据库名遍历器
    private static class DatabaseNameVisitor extends MySQLStatementSQLVisitor {
        private final Map<String, String> mapping;
        private final Set<StringUtils.ReplaceInfo> replaces = new HashSet<>();

        DatabaseNameVisitor(Map<String, String> mapping) {
            this.mapping = mapping;
        }

        @Override
        public ASTNode visit(ParseTree tree) {
            ASTNode node = super.visit(tree);
            if (node instanceof OwnerAvailable) {
                if (((OwnerAvailable) node).getOwner().isPresent()) {
                    visitOwnerSegment(((OwnerAvailable) node).getOwner().get());
                }
            }
            return node;
        }

        private void visitOwnerSegment(OwnerSegment owner) {
            if (owner.getOwner().isPresent()) {
                visitOwnerSegment(owner.getOwner().get());
                return;
            }
            //如果是最顶级的owner,则判断是否需要替换
            String value = owner.getIdentifier().getValue();
            //判断是否存在
            if (mapping.containsKey(value)) {
                replaces.add(new StringUtils.ReplaceInfo(owner.getStartIndex(), owner.getStopIndex() + 1, mapping.get(value)));
            }
        }
    }


    /**
     * 根据mapping将SQL中的数据库名做重映射
     * TODO: 不稳定版本,可能会出现替换错误
     *
     * @param sql     SQL语句
     * @param mapping 数据库名映射
     * @return 返回修改了数据库名后的SQL
     */
    public static String replaceDatabaseName(String sql, Map<String, String> mapping) {
        //解析为AST
        ParseASTNode ast = parse(sql);
        //遍历解析出需要替换的位置
        DatabaseNameVisitor databaseNameVisitor = new DatabaseNameVisitor(mapping);
        ast.getRootNode().accept(databaseNameVisitor);
        //替换字符串的位置
        return StringUtils.replace(sql, new ArrayList<>(databaseNameVisitor.replaces));
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
