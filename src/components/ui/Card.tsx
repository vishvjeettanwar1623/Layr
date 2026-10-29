import React from "react";

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  hover?: boolean;
}

export const Card: React.FC<CardProps> = ({
  children,
  hover = true,
  className = "",
  ...props
}) => {
  return (
    <div
      className={`bg-zinc-900/60 border border-zinc-800 rounded-lg p-4 transition-all duration-150 ${
        hover ? "hover:border-zinc-700 hover:bg-zinc-900/90" : ""
      } ${className}`}
      {...props}
    >
      {children}
    </div>
  );
};

