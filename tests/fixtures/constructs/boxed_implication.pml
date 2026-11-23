/* The same property inside a [], which this project's own emitter produces and which decides. */
byte x = 0;
byte y = 0;

active proctype P() { x = 1; y = 1 }

ltl p0 { [] ( x == 0 -> <> ( y == 1 ) ) }
