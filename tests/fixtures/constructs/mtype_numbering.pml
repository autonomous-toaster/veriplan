/* mtype values are numbered from 1 in reverse declaration order: for { a, b, c } pan gives a = 3, b = 2,
   c = 1. Established here by one pan run per (name, value) pair rather than by reading anything.

   Assigning through a separate statement rather than an initialiser, because an initialised mtype
   declaration is the silent-pass row beside this one. */
mtype = { a, b, c };
mtype x;

active proctype P() { x = a; assert(x == 3) }
