/* Control for unless_second_step.pml: the same body with nothing to preempt it, so c becomes 3 and the same
   claim is refuted. One file accepts the claim and the other refuses it, which is what makes the pair
   discriminating rather than two models that happen to agree. */
byte a = 0;
byte b = 0;
byte c = 0;

active proctype P() { a = 1; b = 2; c = 3 }

ltl p0 { [] ( c == 0 ) }
