import { useEffect, useState } from 'react'
import './App.css'
import { listChat } from './api';

function App() {
  const [texts, setTexts] = useState<string[]>([]);

  useEffect(() => {
    const fetchChat = async () => {
      const chats = await listChat();
      console.log(chats);
    }
    fetchChat();
  }, []);

  const handleSendText = (text: string) => {
    setTexts([...texts, text]);
  };
  return (
    <>
      {
        texts.map((text, index) => {
          return (
            <div key={index}>
              <p>{text}</p>
            </div>
          )
        })
      }

      <ChatInput sendText={handleSendText} />
    </>
  )
}

interface ChatInputProps {
  sendText: (text: string) => void
}

function ChatInput(props: ChatInputProps) {
  const [text, setText] = useState<string>("");
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && e.metaKey) {
      console.log("Send message", text);
      props.sendText(text);
      setText("");
    }
  }

  return (
    <textarea
      className="text-area footer"
      placeholder="Type here..."
      value={text}
      onChange={(e) => setText(e.target.value)}
      onKeyDown={(e) => handleKeyDown(e)}
    ></textarea>
  )
}

export default App
