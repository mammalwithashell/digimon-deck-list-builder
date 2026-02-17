import client from './client';
import type { TokenResponse, User } from '@/types/auth';

export async function register(
  username: string,
  email: string,
  password: string,
): Promise<User> {
  const { data } = await client.post('/auth/register', { username, email, password });
  return data;
}

export async function login(
  username: string,
  password: string,
): Promise<TokenResponse> {
  const { data } = await client.post<TokenResponse>('/auth/login', { username, password });
  return data;
}

export async function refresh(refreshToken: string): Promise<TokenResponse> {
  const { data } = await client.post<TokenResponse>('/auth/refresh', {
    refresh_token: refreshToken,
  });
  return data;
}

export async function logout(refreshToken: string): Promise<void> {
  await client.post('/auth/logout', { refresh_token: refreshToken });
}
