/* A handler of two statements. The escape is armed by the first statement of the body, and the handler
   sets y = 9, so the claim bounding y at 2 is refuted: pan 1.

   An engine that refuses a multi-statement handler gets no verdict here, which is right; one that
   approximates it by running only the first statement of the handler would answer 0 and be caught. */
byte x = 0;
byte y = 0;

active proctype P() {
	do
	:: true -> x = 1; y = 2; break
	od unless { (x == 1); y = 9 }
}

ltl p1 { [] ( y <= 2 ) }
