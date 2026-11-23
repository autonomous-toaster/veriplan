/* The claim never accepts, so pan finds no violation: got is never set, so the automaton's only move is
   its own self-loop. This is the acceptance-cycle oracle - a guard-shaped monitor answers 0 here too,
   but for the wrong reason, having no cycle test at all. */
bool got = false;

active proctype P() { skip }

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
