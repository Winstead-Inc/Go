#include <iostream>
#include <vector>
#include <string>

template<typename T>
class BufferPool
{
    std::vector<T> Items;
    public:
    void AddItem(const T & Item) { Items.push_back(Item); }
};

void ExecuteComplexPipeline(const std::string & SourceName, const std::string & TargetDestination, int TimeoutMilliseconds, bool EnableRetryPolicy)
{
    if (TimeoutMilliseconds > 0) { std::cout << "Starting pipeline" << std::endl; } else { std::cout << "Invalid timeout" << std::endl; }
}
