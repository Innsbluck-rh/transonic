import { Component, JSX } from 'solid-js';

const Heading2: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`archivo-black w-fit origin-top-left scale-x-150 text-lg tracking-tighter ${props.class}`}>
      {props.children}
    </p>
  );
};

export default Heading2;
