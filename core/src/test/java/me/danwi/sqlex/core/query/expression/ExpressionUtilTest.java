package me.danwi.sqlex.core.query.expression;

import me.danwi.sqlex.core.query.SQLParameterBind;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

import static me.danwi.sqlex.core.query.expression.Expression.*;

public class ExpressionUtilTest {
    @Test
    public void toSQL() {
        Expression expression = arg(true).and(arg(false));
        SQLParameterBind expressionBindResult = ExpressionUtil.toSQL(expression);
        assertEquals(expressionBindResult.getSQL(), "(?) and (?)");
        assertEquals(expressionBindResult.getParameters().get(0), true);
        assertEquals(expressionBindResult.getParameters().get(1), false);
    }
}
