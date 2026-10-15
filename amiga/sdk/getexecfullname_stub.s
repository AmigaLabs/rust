	.section ".text"
	.align 2
	.globl getexecfullname
	.type getexecfullname, @function
getexecfullname:
	li %r12, 4524
	b __Clib4Call
	.size getexecfullname, .-getexecfullname
