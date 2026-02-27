// Arrow function exports
export const fetchUser = async (id: string): Promise<User> => {
  const response = await fetch(`/api/users/${id}`);
  return response.json();
};

export const add = (a: number, b: number): number => a + b;

const internalHelper = (x: string) => {
  return x.trim();
};

// Overloaded functions
function process(input: string): string;
function process(input: number): number;
function process(input: string | number): string | number {
  if (typeof input === 'string') return input.trim();
  return input * 2;
}

export function format(value: string): string;
export function format(value: number): string;
export function format(value: string | number): string {
  return String(value);
}
