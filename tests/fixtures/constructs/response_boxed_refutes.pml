/* A response property under a box, with the response never delivered: a eventually holds and b never does,
   so [] ( a -> <> b ) is refuted. pan 1, and the engine decides 1. */
byte a = 0;
byte b = 0;

active proctype P() { a = 1 }

ltl p0 { [] ( a == 1 -> <> ( b == 1 ) ) }
