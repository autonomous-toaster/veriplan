/* The claim's assert never fires: got is never set. pan 0, and it would be 0 for a completion-only
   reading too, so this file is the weaker half of the pair. */
bool got = false;

active proctype P() { skip }

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
