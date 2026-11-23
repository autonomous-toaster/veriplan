/* The claim names _nr_pr at a point where a child has started and has not finished.

   This is the only shape that keeps the process count in the state honest: a child started with run()
   blocks on a rendezvous, so it is alive while the parent asserts, and a statically expanded fixed bound
   would also pass. An engine that does not carry the count, or carries it without the child, answers 1. */
chan done = [0] of { bit };

proctype Child() { done ? 1 }

active proctype Parent() {
	run Child();
	assert(_nr_pr == 2);
	done ! 1
}
