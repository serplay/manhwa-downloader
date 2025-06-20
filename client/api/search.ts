export async function GET(request: Request) {
    const { searchParams } = new URL(request.url);
  
    const title = searchParams.get("title");
    const source = searchParams.get("source");
  
    if (!title || !source) {
      return new Response(
        JSON.stringify({ error: "Missing required query parameters: title and source" }),
        { status: 400, headers: { "Content-Type": "application/json" } }
      );
    }
  
    const apiUrl = `https://mangapdf.losingsanity.com/search?title=${encodeURIComponent(title)}&source=${encodeURIComponent(source)}`;
  
    try {
      const response = await fetch(apiUrl);
      const data = await response.json();
      return new Response(JSON.stringify(data), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    } catch (error) {
      return new Response(
        JSON.stringify({ error: "Failed to fetch data from external API" }),
        { status: 500, headers: { "Content-Type": "application/json" } }
      );
    }
  }
  