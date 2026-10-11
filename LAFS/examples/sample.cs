using System;
using System.Collections.Generic;

namespace ExampleNamespace
{
    public class DataProcessor
    {
        public List<string> Items;

        public void ExecutePipeline(string Payload) { if (Payload != null) { Console.WriteLine("Processing payload"); } }
    }
}
