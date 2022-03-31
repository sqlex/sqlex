lexer grammar SqlExMethodLanguageLexer;

WS: [ \t\r\n]+ -> channel(HIDDEN);
COMMENT: [#!](~[\r\n])*;

IMPORT: 'import';

ID: [a-zA-Z]([_a-zA-Z0-9])*;
LCB: '{' -> pushMode(SQL);

DOT: '.';
STAR: '*';
LB: '(';
RB: ')';
COLON: ':';
COMMA: ',';

ERRCHAR: . -> channel(HIDDEN);

mode SQL;
SQL_WS: WS -> channel(HIDDEN);
RCB: '}' -> popMode;
SQL_TEXT: FIRST_SQL_CHAR SQL_CHAR+;

fragment FIRST_SQL_CHAR: ~[{} \n\t\f];
fragment SQL_CHAR: ~[{}] | '\\'[{}];