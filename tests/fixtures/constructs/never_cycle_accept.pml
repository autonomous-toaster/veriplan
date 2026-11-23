/* An accept state that is reachable and cyclic: the claim can spin in accept_x forever regardless of what
   the model does, so the verdict here is an acceptance cycle and not an assertion.

   The model terminates, which is the point - there is no model behaviour to observe, so a wrong
   encoding that looks for completion or for a claim assertion has nothing to find. */
byte x = 0;

active proctype P() { x = 1 }

never {
accept_x:
	do
	:: (1) -> goto accept_x
	od
}
