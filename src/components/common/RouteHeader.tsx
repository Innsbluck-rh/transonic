import { Component, JSX } from 'solid-js';

interface RouteHeaderProps {
  title: string;
  children?: JSX.Element;
}

const RouteHeader: Component<RouteHeaderProps> = (props) => {
  return (
    <div class='border-primary-text/35 sticky top-0 z-60 flex h-12 min-h-12 w-full items-end overflow-hidden border-b backdrop-blur-sm'>
      <p class={`archivo-black z-50 -mb-2 -ml-1 w-fit origin-top-left scale-x-200 text-3xl tracking-tighter opacity-35`}>{props.title}</p>
      {props.children}
    </div>
  );
};

export default RouteHeader;
