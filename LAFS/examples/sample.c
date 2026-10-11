#include <stdio.h>

int ComputeSum(int A, int B) { return A + B; }

void ProcessAccountData(int AccountId, const char * SecurityToken)
{
    if (AccountId > 0) { printf("Valid account\n"); } else { printf("Invalid account\n"); }
}
