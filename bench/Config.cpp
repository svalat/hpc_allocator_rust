/****************************************************/
#include <argp.h>
#include <array>
#include <memory>
#include <cassert>
#include <iostream>
#include "Config.hpp"

/****************************************************/
using namespace allocbench;
using namespace std;

/****************************************************/
static const char * cstBenchModes[] = {"fixed", "list", "rand", "trace"};
static const char * cstReuseModes[] = {"linear", "full", "rand"};

/****************************************************/
const char *argp_program_version = "alloc-bench 1.0";
const char *argp_program_bug_address = "<sebastien.valat-dev@orange.fr>";
static char doc[] = "A simple memory allocator benchmark.";
static char args_doc[] = "";
static struct argp_option options[] = {
	{"bench",      'b', "MODE",       0, "Benchmark running mode: 'fixed', 'list', 'rand', 'trace'." },
	{"reuse",      'r', "MODE",       0, "Slot reuse: 'linear', 'full', 'rand'."},
	{"size",       's', "SIZE",       0, "Size to be used. In fix mode only one value, in list mode a comma separated list and in rand mode a min, step, max comma separated list."},
	{"keep",       'k', "KEEP",       0, "Number of allocation to keep alive at the same time."},
	{"iterations", 'i', "ITERATIONS", 0, "Number of iterations to make."},
	{"memset",     'm', 0,            0, "Activate call and measurement of memset on the segment."},
	{"no-perf",    'n', 0,            0, "Disable internal perf measurement."},
	{"quiet",      'q', 0,            0, "Do not print the benchmark header with system info."},
	{"progress",   'p', 0,            0, "Display a progress bar."},
	{"cache",      'c', "SIZE",       0, "Maximal cache size in GB (floating point)."},
	{"trace",      't', "FILE",       0, "Trace file to be used when bench mode is 'trace'."},
	{ 0 }
};

/****************************************************/
Config::Config(void)
{
	this->bench = BENCH_FIXED;
	this->reuse = REUSE_LINEAR;
	this->sizes.push_back(256);
	this->keep = 1024;
	this->iterations = 500000;
	this->memset = false;
	this->perf = true;
	this->quiet = false;
	this->progress = false;
	this->opCache = 1024;
}

/****************************************************/
static std::string exec(const char* cmd) {
    std::array<char, 128> buffer;
    std::string result;
    std::shared_ptr<FILE> pipe(popen(cmd, "r"), pclose);
    if (!pipe) throw std::runtime_error("popen() failed!");
    while (!feof(pipe.get())) {
        if (fgets(buffer.data(), 128, pipe.get()) != nullptr)
            result += buffer.data();
    }
    return result;
}

/****************************************************/
void Config::print(void)
{
	const char * env = getenv("LD_PRELOAD");
	std::string ldPreload;
	if (env != NULL)
		ldPreload = env;
	unsetenv("LD_PRELOAD");
	cout << "###################  SOURCE  ###################" << endl;
	cout << "#" << endl;
	cout << "# Date: " << exec("date +%c");
	cout << "# Hash: " << exec("git rev-parse HEAD");
	cout << "#" << endl;
	cout << "###################  SYSTEM  ###################" << endl;
	cout << "#" << endl;
	cout << "# Gcc: " << exec("gcc --version | head -n 1");
	cout << "# G++: " << exec("g++ --version | head -n 1");
	cout << "# Rustc: " << exec("rustup run nightly rustc --version || echo 'No rust'");
	cout << "# Kernel: " << exec("uname -a");
	cout << "# Processor: " << exec("cat /proc/cpuinfo | grep 'model name' | head -n 1");
	cout << "# LD_PRELOAD: " << ldPreload << endl;
	cout << "#" << endl;
	cout << "###################  CONFIG  ###################" << endl;
	cout << "#" << endl;
	cout << "# Bench: " << cstBenchModes[this->bench] << endl;
	cout << "# Reuse: " << cstReuseModes[this->reuse] << endl;
	cout << "# Sizes: ";
	for (auto & it: this->sizes)
		cout << it << ", ";
	cout << endl;
	cout << "# Trace: " << this->trace << endl;
	cout << "# Keep: " << this->keep << endl;
	cout << "# Iterations: " << this->iterations << endl;
	cout << "# Memset: " << (this->memset?"true":"false") << endl;
	cout << "# Perf: " << (this->perf?"true":"false") << endl;
	cout << "# OpCache: " << this->opCache << " GB" << endl;
	cout << "#" << endl;
	cout << "####################  DATA  ####################" << endl;
}

/****************************************************/
BenchMode getBenchMode(const std::string & value)
{
	if (value == "fixed")
		return BENCH_FIXED;
	else if (value == "list")
		return BENCH_LIST;
	else if (value == "rand")
		return BENCH_RAND;
	else if (value == "trace")
		return BENCH_TRACE;
	else
		assert(false);
}

/****************************************************/
ReuseMode getReuseMode(const std::string & value)
{
	if (value == "linear")
		return REUSE_LINEAR;
	else if (value == "full")
		return REUSE_FULL;
	else if (value == "rand")
		return REUSE_RAND;
	else
		assert(false);
}

/****************************************************/
static std::vector<size_t> splitToVector(const std::string & value, char separator)
{
	//vars
	std::vector<size_t> output;
	std::string::size_type prev_pos = 0, pos = 0;

	//loop on separators
	while((pos = value.find(separator, pos)) != std::string::npos)
	{
			std::string substring(value.substr(prev_pos, pos-prev_pos));
			output.push_back(atol(substring.c_str()));
			prev_pos = ++pos;
	}

	//push last
	output.push_back(atol(value.substr(prev_pos, pos-prev_pos).c_str()));

	//return
	return output;
}

/****************************************************/
/* Parse a single option. */
static error_t parseOptions (int key, char *arg, struct argp_state *state)
{
	//get args
	struct Config *config = static_cast<Config*>(state->input);

	switch (key)
	{
		case 'b':
			config->bench = getBenchMode(arg);
			break;
		case 'r':
			config->reuse = getReuseMode(arg);
			break;
		case 's':
			config->sizes = splitToVector(arg, ',');
			break;
		case 'k':
			config->keep = atol(arg);
			break;
		case 'i':
			config->iterations = atol(arg);
			break;
		case 'm':
			config->memset = true;
			break;
		case 'n':
			config->perf = false;
			break;
		case 'q':
			config->quiet = true;
			break;
		case 'p':
			config->progress = true;
			break;
		case 'c':
			config->opCache = atof(arg);
			break;
		case 't':
			config->trace = arg;
			break;
		case ARGP_KEY_ARG:
			argp_usage (state);
			break;
		case ARGP_KEY_END:
			break;
		default:
			return ARGP_ERR_UNKNOWN;
	}
	return 0;
}

/****************************************************/
/* Our argp parser. */
static struct argp argp = { options, parseOptions, args_doc, doc };

/****************************************************/
void Config::parse(int argc, char ** argv)
{
	argp_parse (&argp, argc, argv, 0, 0, this);
}
