export const newChat = async (message: string) => {
	try {
		const response = await fetch("/api/chat", {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify({ message }),
		});
		const data = await response.json();
		console.log(data);
	} catch (e) {
		console.log("ERROR:", e);
	}
};

export const updateChat = async (chatId: string, message: string) => {
	try {
		const response = await fetch(`/api/chat/${chatId}`, {
			method: "PUT",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify({ message }),
		});
		const data = await response.json();
		console.log(data);
	} catch (e) {
		console.log("ERROR:", e);
	}
};

export const listChat = async () => {
	try {
		const response = await fetch("/api/chat");
		const data = await response.json();
		console.log(data);
	} catch (e) {
		console.log("ERROR:", e);
	}
};
