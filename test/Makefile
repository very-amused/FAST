src-tests=test1-stream.c
bin-tests=$(src-tests:.c=)
libfast=../target/debug/libfast.a
override CFLAGS = -O1 -I../include/

# Prefer building with clang
ifneq (,$(shell which clang))
	CC=clang
else
	CC=gcc
endif

define compile-test
$2: $1
	clang $$(CFLAGS) -o$$@ $$< $(libfast)
endef

all=$(src-tests)

# Generate test rules
$(foreach f,$(src-tests),$(eval $(call compile-test,$(f),$(f:.c=))))

clean: $(bin-tests)
	rm $<
.PHONY: clean
