/* A comparison atom inside a liveness formula. The same atom in a safety claim is decided, so a decline
   here is a gap in the liveness path rather than in atom evaluation.

   pan: 0, plain and -a. The claim HOLDS - x = 1 runs, so eventually x == 1 - and a first version of this
   comment said "refuted, because x becomes 1", which is the safety claim [] ( x == 1 ) read onto a
   liveness formula. The table beside it always recorded pan 0; the prose did not, and a message sent with
   the wrong number made a correct engine look like a regression until the producer measured it. */
byte x = 0;

active proctype P() { x = 1 }

ltl p0 { <> ( x == 1 ) }
