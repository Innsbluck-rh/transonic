import { Component, JSX } from 'solid-js';

const Heading1: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`archivo-black w-fit origin-top-left scale-x-150 text-2xl font-black tracking-tighter ${props.class}`}>
      {props.children}
    </p>
  );
};

export default Heading1;
