import React from 'react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary';
  size?: 'sm' | 'md' | 'lg';
}

export const Button: React.FC<React.PropsWithChildren<ButtonProps>> = ({
  variant,
  size,
  children,
  ...props
}) => {
  return (
    <button
      className="bg-slate-900 text-slate-100 rounded-sm px-4 py-2"
      {...props}
    >
      {children}
    </button>
  );
};
