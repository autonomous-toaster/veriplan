/* An implication with a liveness consequent and no outer [] - the shape spin's own petersonN uses. */
byte x = 0;
byte y = 0;

active proctype P() { x = 1; y = 1 }

ltl p0 { x == 0 -> <> ( y == 1 ) }
