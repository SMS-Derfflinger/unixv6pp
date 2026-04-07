binary := fseditor

.DEFAULT_TARGET = $(binary)

CXX := g++

CXXFLAGS := -O -I$(CURDIR) -std=c++20
LDFLAGS :=

objs := FileSystemAdapter.o structures/SuperBlock.o structures/Inode.o structures/InodeDirectory.o main.o
hdrs := $(wildcard $(CURDIR)/*.h)

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

$(binary): $(objs) $(hdrs)
	echo $(hdrs)
	$(CXX) $(LDFLAGS) -o $@ $(objs)

.PHONY: clean
clean:
	-rm -f $(objs)
	-rm -f $(binary)
