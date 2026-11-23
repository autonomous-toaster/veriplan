/* Control for nrpr_child_alive.pml: the same model claiming one process instead of two, which is refuted
   when the count is right. One file holds and the other fails, so the pair distinguishes a count that is
   read from a count that is guessed.

   Note on a shape that looks equivalent and is not: _nr_pr inside an ltl claim gave pan 1 here even with
   two processes, so it is not usable as an oracle until that is understood. The assert form is. */
chan done = [0] of { bit };

proctype Child() { done ? 1 }

active proctype Parent() {
	run Child();
	assert(_nr_pr == 1);
	done ! 1
}
