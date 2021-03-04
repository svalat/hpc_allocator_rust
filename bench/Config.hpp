#ifndef BENCH_CONFIG_HPP
#define BENCH_CONFIG_HPP

/****************************************************/
#include <cstdlib>
#include <string>
#include <vector>

/****************************************************/
namespace allocbench
{

/****************************************************/
enum BenchMode
{
	BENCH_FIXED,
	BENCH_LIST,
	BENCH_RAND,
	BENCH_TRACE,
};

/****************************************************/
enum ReuseMode
{
	REUSE_LINEAR,
	REUSE_FULL,
	REUSE_RAND,
};

/****************************************************/
/* Used by main to communicate with parse_opt. */
struct Config
{
	//functions
	Config(void);
	void print(void);
	void parse(int argc, char ** argv);
	//members
	BenchMode bench;
	ReuseMode reuse;
	std::vector<size_t> sizes;
	std::string trace;
	size_t keep;
	size_t iterations;
	float opCache;
	bool memset;
	bool perf;
	bool quiet;
	bool progress;
};

}

#endif //BENCH_CONFIG_HPP
