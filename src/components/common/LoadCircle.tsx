import { Component } from 'solid-js';

interface LoadCircleProps {
  class?: string;
  // replaces the default size (font-size plus the margin that keeps the 1rem layout box)
  sizeClass?: string;
}

const LoadCircle: Component<LoadCircleProps> = (props) => {
  return <i class={`icon-[line-md--loading-twotone-loop] text-primary-text ${props.sizeClass ?? '-m-2 text-[32px]'} ${props.class}`} />;
};

export default LoadCircle;
