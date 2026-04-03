binary := fseditor

.DEFAULT_TARGET = $(binary)

CXX := clang++

CXXFLAGS := -O -I$(PWD) -std=c++20
LDFLAGS :=

objs := FileSystemAdapter.o structures/SuperBlock.o structures/Inode.o structures/InodeDirectory.o main.o
hdrs := $(wildcard $(PWD)/*.h)

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

$(binary): $(objs) $(hdrs)
	echo $(hdrs)
	$(CXX) $(LDFLAGS) -o $@ $(objs)

.PHONY: clean
clean:
	-rm -f $(objs)
	-rm -f $(binary)
