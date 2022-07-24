package me.danwi.sqlex.core.query.expression;

import me.danwi.sqlex.core.query.SQLParameterBind;
import org.junit.jupiter.api.Test;

import static me.danwi.sqlex.core.query.expression.Expression.*;
import static me.danwi.sqlex.core.query.expression.Expression.lit;
import static org.junit.jupiter.api.Assertions.assertEquals;

public class FunExpressionTest {

    @Test
    public void concatTest() {
        FunctionCallExpression concat = concat(new ParameterExpression("b"), new ParameterExpression("v"));
        SQLParameterBind sqlResult = ExpressionUtil.toSQL(concat);
        assertEquals(sqlResult.getSQL(), "concat(?,?)");
    }

    @Test
    public void concatWsTest() {
        FunctionCallExpression concat = concatWs(lit(","), new ParameterExpression("a"),new ParameterExpression("b"));
        SQLParameterBind sqlResult = ExpressionUtil.toSQL(concat);
        assertEquals(sqlResult.getSQL(), "concat_ws(',',?,?)");
    }

}
