import { Link, useNavigate } from 'react-router-dom';
import { LoginForm } from '@/components/auth/LoginForm';

export function LoginPage() {
  const navigate = useNavigate();

  return (
    <div className="flex flex-col items-center justify-center min-h-[calc(var(--app-vh100,100vh)-56px)] p-8">
      <h1 className="text-2xl font-bold text-gray-100 mb-6">Log In</h1>
      <LoginForm onSuccess={() => navigate('/')} />
      <p className="mt-4 text-sm text-gray-400">
        Don't have an account?{' '}
        <Link to="/register" className="text-blue-400 hover:text-blue-300">
          Register
        </Link>
      </p>
    </div>
  );
}
