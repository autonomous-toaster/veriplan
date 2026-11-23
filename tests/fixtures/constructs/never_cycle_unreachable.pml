/* The control: an accept state no run can reach, so there is no accepting cycle and pan reports nothing.
   Same model, same shape, only the guard on the way in differs. */
byte x = 0;

active proctype P() { x = 1 }

never {
T0_init:
	do
	:: (x == 7) -> goto accept_x
	:: (1) -> goto T0_init
	od;
accept_x:
	skip
}
