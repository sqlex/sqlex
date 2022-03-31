parser grammar SqlExMethodLanguageParser;

options { tokenVocab=SqlExMethodLanguageLexer;}

root: (importEx|method|COMMENT)*? EOF;

importEx: IMPORT className;

className: (ID DOT)* (ID|STAR);

method: returnType? methodName LB paramList? RB LCB sql RCB;

paramList: (param COMMA)* param;

param: paramName COLON paramType paramRepeat?;

paramName: ID;

paramType: (ID DOT)* ID;

paramRepeat: STAR;

returnType: ID;

methodName: ID;

sql: SQL_TEXT;