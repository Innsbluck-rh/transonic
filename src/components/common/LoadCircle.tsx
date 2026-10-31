import { Component } from 'solid-js';

interface LoadCircleProps {
  class?: string;
}

const LoadCircle: Component<LoadCircleProps> = (props) => {
  return <i class={`icon-[line-md--loading-twotone-loop] text-primary-text scale-200 ${props.class}`} />;
};

export default LoadCircle;
