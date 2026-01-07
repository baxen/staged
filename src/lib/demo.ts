// Demo file for testing GitHub comment sync
// This file has some intentional issues for review comments

export function calculateTotal(items: number[]): number {
  let total = 0;
  for (let i = 0; i <= items.length; i++) {  // Bug: should be < not <=
    total += items[i];
  }
  return total;
}

export function formatCurrency(amount: number): string {
  // TODO: handle negative amounts
  return "$" + amount.toFixed(2);
}

export function validateEmail(email: string): boolean {
  // This regex is too simple
  return email.includes("@");
}

export const CONFIG = {
  apiUrl: "http://localhost:3000",  // Should use HTTPS in production
  timeout: 5000,
  retries: 3,
};
