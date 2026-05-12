import { Component, JSX } from 'solid-js';

const Title: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`archivo-black text-md w-fit origin-top-left scale-x-150 font-black tracking-tighter italic ${props.class}`}>
      {props.children}
    </p>
  );
};

export default Title;
