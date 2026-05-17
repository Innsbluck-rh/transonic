import { Component, JSX } from 'solid-js';

interface RouteHeaderProps {
  title: string;
  children?: JSX.Element;
}

const RouteHeader: Component<RouteHeaderProps> = (props) => {
  return (
    <div class='border-primary-border flex h-12 min-h-12 w-full items-end overflow-hidden border-b'>
      <p class={`archivo-black -mb-2 w-fit origin-top-left scale-x-150 text-4xl tracking-tighter opacity-20`}>{props.title}</p>
      {props.children}
    </div>
  );
};

export default RouteHeader;
