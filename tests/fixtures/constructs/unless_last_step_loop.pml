/* The same arming as unless_last_step_block.pml, but the body is a loop: the escape is re-checked at the
   loop head with x already 3, so the handler does run and the claim is refuted. pan 1.

   This pair exists because the first two unless models only fixed the boundary at the body's first step,
   and because a block and a loop with the same handler disagree - which is the kind of thing a description
   cannot convey and a file can. */
byte x = 0;
byte y = 0;

active proctype P() {
	do
	:: true -> x = 1; x = 2; x = 3
	od unless { (x == 3); y = 9 }
}

ltl p0 { [] ( y <= 2 ) }
