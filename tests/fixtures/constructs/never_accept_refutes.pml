/* The claim's assert fires: the rendezvous handshake sets got. pan: "assertion violated !(got)",
   errors 1 - which is the claim assertion, not an acceptance cycle. */
chan c = [0] of { bit };
bool got = false;

active proctype Sender() { c ! 1 }
active proctype Receiver() { c ? got }

/* spin -f '<> (got)' printed this automaton verbatim. spin -f emits the NEGATION automaton, so the
   file where the property holds is the one pan refutes - naming by the property inverts the oracle.

   Correction, because this file first claimed otherwise: this is NOT an acceptance-cycle test. pan's
   verdict on the refuting file is the claim's own "assertion violated !(got)", and `accept_all` is
   unreachable because the do-loop above it never terminates, so no accepting cycle exists. What the
   pair exercises is whether a claim's assert can fire. A cycle test needs never_cycle_accept.pml. */
never  {    /* <> (got) */
T0_init:
	do
	:: atomic { ((got)) -> assert(!((got))) }
	:: (1) -> goto T0_init
	od;
accept_all:
	skip
}
