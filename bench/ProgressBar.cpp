/****************************************************/
#include <iostream>
#include "ProgressBar.hpp"

/****************************************************/
using namespace allocbench;
using namespace std;

/****************************************************/
ProgressBar::ProgressBar(size_t size, size_t max, bool enabled)
{
	this->size = size;
	this->max = max - 1;
	this->enabled = enabled;
	this->buffer = new char[size+1];
	this->buffer[size] = '\0';
	this->cur = 0;
}

/****************************************************/
ProgressBar::~ProgressBar(void)
{
	delete [] this->buffer;
	if (enabled)
		cerr << endl;
}

/****************************************************/
void ProgressBar::progress(size_t value)
{
	//disabled
	if (enabled == false)
		return;
	
	//apply
	size_t new_cur = (size * value) / max;
	if (cur != new_cur) {
		for (size_t i = 0 ; i < size ; i++)
			this->buffer[i] = (i <= cur)?'=':'.';
		int progress = (100 * value) / max;
		cerr << '\r' << '[' << this->buffer << "] " << progress << "%" << std::flush;
		this->cur = new_cur;
	}
}
