import { Icon } from '@iconify-icon/solid';
import { Component } from 'solid-js';

interface LoadCircleProps {
  class?: string;
}

const LoadCircle: Component<LoadCircleProps> = (props) => {
  return <Icon icon='line-md:loading-twotone-loop' class={`text-primary-text scale-200 ${props.class}`} />;
};

export default LoadCircle;
