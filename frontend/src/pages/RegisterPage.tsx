import { Link, useNavigate } from 'react-router-dom';
import { RegisterForm } from '@/components/auth/RegisterForm';

export function RegisterPage() {
  const navigate = useNavigate();

  return (
    <div className="flex flex-col items-center justify-center min-h-[calc(100vh-56px)] p-8">
      <h1 className="text-2xl font-bold text-gray-100 mb-6">Create Account</h1>
      <RegisterForm onSuccess={() => navigate('/login')} />
      <p className="mt-4 text-sm text-gray-400">
        Already have an account?{' '}
        <Link to="/login" className="text-blue-400 hover:text-blue-300">
          Log In
        </Link>
      </p>
    </div>
  );
}
