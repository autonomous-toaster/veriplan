/* The same handler shape as unless_multi_handler, but the escape is armed by the body's SECOND statement
   rather than its first. A handler counter that is off by one at a later step lets the body run on to its
   third statement, so c becomes 3 and the claim is refuted; a correct preemption stops before it.

   One claim only, and deliberately not a second one about d: [] ( d == 9 ) would be refuted in the initial
   state, where d is still 0, which says nothing about the handler.

   Paired with unless_second_step_nohandler.pml, the same body with no handler at all. */
byte a = 0;
byte b = 0;
byte c = 0;
byte d = 0;

active proctype P() {
	do
	:: true -> a = 1; b = 2; c = 3
	od unless { (b == 2); d = 9 }
}

ltl p0 { [] ( c == 0 ) }
