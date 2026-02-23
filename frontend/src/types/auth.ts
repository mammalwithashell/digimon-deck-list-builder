export interface User {
  id: string;
  username: string;
  email: string;
  display_name?: string | null;
  avatar_url?: string | null;
  roles: string[];
}

export interface TokenResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface RegisterRequest {
  username: string;
  email: string;
  password: string;
}
