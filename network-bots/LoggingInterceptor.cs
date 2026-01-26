using Grpc.Core;
using Grpc.Core.Interceptors;

namespace MiniGameNetworkBot;

public sealed class LoggingInterceptor : Interceptor
{
    public override AsyncDuplexStreamingCall<TRequest, TResponse> AsyncDuplexStreamingCall<TRequest, TResponse>(
        ClientInterceptorContext<TRequest, TResponse> context,
        AsyncDuplexStreamingCallContinuation<TRequest, TResponse> continuation)
    {
        var call = continuation(context);

        var loggingRequestStream = new LoggingRequestStreamWriter<TRequest>(call.RequestStream);

        var loggingResponseStream = new LoggingResponseStreamReader<TResponse>(call.ResponseStream, call.GetStatus);

        return new AsyncDuplexStreamingCall<TRequest, TResponse>(
            loggingRequestStream,
            loggingResponseStream,
            call.ResponseHeadersAsync,
            call.GetStatus,
            call.GetTrailers,
            call.Dispose);
    }
    
    private sealed class LoggingRequestStreamWriter<TRequest>: IClientStreamWriter<TRequest>
    {
        private readonly IClientStreamWriter<TRequest> _callRequestStream;

        public LoggingRequestStreamWriter(IClientStreamWriter<TRequest> callRequestStream)
        {
            _callRequestStream = callRequestStream;
        }

        public Task WriteAsync(TRequest message)
        {
            Console.WriteLine(">>> {0}", message);
            return _callRequestStream.WriteAsync(message);
        }

        public WriteOptions? WriteOptions
        {
            get => _callRequestStream.WriteOptions;
            set => _callRequestStream.WriteOptions = value;
        }
        public Task CompleteAsync()
        {
            return _callRequestStream.CompleteAsync();
        }
    }
    
    private sealed class LoggingResponseStreamReader<TResponse> : IAsyncStreamReader<TResponse>
    {
        private readonly IAsyncStreamReader<TResponse> _callResponseStream;
        private readonly Func<Status> _getStatus;

        public LoggingResponseStreamReader(IAsyncStreamReader<TResponse> callResponseStream, Func<Status> getStatus)
        {
            _callResponseStream = callResponseStream;
            _getStatus = getStatus;
        }

        public TResponse Current => _callResponseStream.Current;

        public async Task<bool> MoveNext(CancellationToken cancellationToken)
        {
            var hasNext = await _callResponseStream.MoveNext(cancellationToken);
            if (hasNext)
            {
                Console.WriteLine("<<< {0}", _callResponseStream.Current);
            }
            else
            {
                var status = _getStatus();
                Console.WriteLine("<<< Call completed with status: {0}", status);
            }
            return hasNext;
        }
    }
}