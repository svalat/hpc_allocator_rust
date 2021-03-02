#ifndef PROGRESS_BAR_HPP
#define PROGRESS_BAR_HPP

/****************************************************/
#include <cstdlib>

/****************************************************/
namespace allocbench
{

/****************************************************/
class ProgressBar
{
	public:
		ProgressBar(size_t size, size_t max, bool enabled);
		~ProgressBar(void);
		void progress(size_t value);
	private:
		char * buffer;
		size_t size;
		size_t max;
		size_t cur;
		bool enabled;
};


}

#endif //PROGRESS_BAR_HPP