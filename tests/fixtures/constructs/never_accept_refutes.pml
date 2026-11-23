/* The claim accepts: the rendezvous handshake completes, got becomes true, and the automaton reaches the
   state pan is looking for. pan: errors 1.

   Paired with never_accept_holds.pml. Both carry the same generated automaton; only the model differs,
   which is what separates a real acceptance condition from a reading of completion. */
chan c = [0] of { bit };
bool got = false;

active proctype Sender() { c ! 1 }
active proctype Receiver() { c ? got }

/* spin -f '<> (got)' printed this automaton verbatim. Note what that means: spin -f emits the
   NEGATION automaton, so pan reports a violation on the run where the claim *accepts* - which for this
   automaton is the run where got becomes true. Getting that backwards makes an oracle that is exactly
   inverted, which is why both files carry pan's measured answer in a comment rather than the property
   name. */
never  {    /* <> (got) */
T0_init:
	do
	:: atomic { ((got)) -> assert(!((got))) }
	:: (1) -> goto T0_init
	od;
accept_all:
	skip
}
