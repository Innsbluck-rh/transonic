import { Component, JSX } from 'solid-js';

const ErrorMsg: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`text-red-500 ${props.class}`}>
      {props.children}
    </p>
  );
};

export default ErrorMsg;
