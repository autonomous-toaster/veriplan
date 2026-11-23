/* A comparison atom inside a liveness formula. The same atom in a safety claim is decided, so a decline
   here is a gap in the liveness path rather than in atom evaluation: pan decides it (refuted, because x
   becomes 1) and an engine that declines gives no verdict. */
byte x = 0;

active proctype P() { x = 1 }

ltl p0 { <> ( x == 1 ) }
