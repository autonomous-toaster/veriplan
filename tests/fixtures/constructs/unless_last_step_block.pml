/* The escape is armed by the body's LAST statement, and the body is a block rather than a loop. Once the
   block has completed there is nothing left to preempt, so the handler never runs: y stays 0 and the claim
   holds. pan 0.

   Paired with unless_last_step_loop.pml, where the same arming fires - the distinction is the shape of the
   body, not the position of the arming statement, and an engine that treats them alike answers 1 on one of
   the two. */
byte x = 0;
byte y = 0;

active proctype P() {
	{ x = 1; x = 2; x = 3 } unless { (x == 3); y = 9 }
}

ltl p0 { [] ( y <= 2 ) }
